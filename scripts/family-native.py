#!/usr/bin/env python3
"""OS primitives for lock recovery; standard library only, no package bootstrap."""
import ctypes
import fcntl
import errno
import json
import os
from pathlib import Path
import stat
import subprocess
import sys


def regular(fd):
    if not stat.S_ISREG(os.fstat(fd).st_mode):
        raise RuntimeError("refusing non-regular recovery lock")


def exchange(left, right):
    # Directory descriptors anchor the syscall; neither file is overwritten or unlinked.
    descriptors = []
    try:
        for value in (left, right):
            parent = os.open(str(Path(value).parent), os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
            descriptors.append(parent)
            if not stat.S_ISREG(os.stat(Path(value).name, dir_fd=parent, follow_symlinks=False).st_mode):
                raise RuntimeError("atomic exchange requires regular files")
        libc = ctypes.CDLL(None, use_errno=True)
        arguments = (descriptors[0], os.fsencode(Path(left).name), descriptors[1], os.fsencode(Path(right).name), 2)
        if sys.platform == "linux":
            rename = libc.renameat2  # RENAME_EXCHANGE
        elif sys.platform == "darwin":
            rename = libc.renameatx_np  # RENAME_SWAP
        else:
            raise RuntimeError("atomic lock exchange is unsupported on this platform")
        rename.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
        rename.restype = ctypes.c_int
        if rename(*arguments) != 0:
            code = ctypes.get_errno()
            raise OSError(code, os.strerror(code))
        for descriptor in descriptors:
            os.fsync(descriptor)
    finally:
        for descriptor in reversed(descriptors):
            os.close(descriptor)


def darwin_process(pid):
    # Darwin's public PROC_PIDTBSDINFO includes kernel start seconds and
    # microseconds; a coarse ps timestamp cannot disambiguate PID reuse.
    if sys.platform != "darwin":
        raise RuntimeError("Darwin process identity requested on another platform")
    class BsdInfo(ctypes.Structure):
        _fields_ = [("prefix", ctypes.c_uint32 * 12),
                    ("comm", ctypes.c_char * 16), ("name", ctypes.c_char * 32),
                    ("details", ctypes.c_uint32 * 6),
                    ("start_seconds", ctypes.c_uint64), ("start_microseconds", ctypes.c_uint64)]
    lib = ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True)
    query = lib.proc_pidinfo
    query.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_uint64, ctypes.c_void_p, ctypes.c_int]
    query.restype = ctypes.c_int
    info = BsdInfo()
    result = query(pid, 3, 0, ctypes.byref(info), ctypes.sizeof(info))
    if result != ctypes.sizeof(info):
        code = ctypes.get_errno()
        if result == 0 and code == errno.ESRCH:
            return {"stopped": True}
        raise RuntimeError(f"cannot establish Darwin process identity: result={result}, errno={code}")
    if info.prefix[3] != pid or not info.start_seconds:
        raise RuntimeError("invalid Darwin process identity")
    return {"startTime": f"{info.start_seconds}:{info.start_microseconds}"}


def recover(lock, node, module, hub, command):
    # This permanent inode is never deleted or replaced. flock survives parent
    # interruption while the recovery child retains the inherited descriptor.
    fd = os.open(lock, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    try:
        regular(fd)
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        current = os.stat(lock, follow_symlinks=False)
        held = os.fstat(fd)
        if (current.st_dev, current.st_ino) != (held.st_dev, held.st_ino):
            raise RuntimeError("recovery lock inode changed")
        env = dict(os.environ, JANKURAI_RECOVERY_FD=str(fd), JANKURAI_RECOVERY_PARENT=str(os.getpid()))
        result = subprocess.run([node, module, "--recovery-worker", hub, command], env=env, pass_fds=(fd,))
        return result.returncode
    finally:
        os.close(fd)


if __name__ == "__main__":
    try:
        if len(sys.argv) == 3 and sys.argv[1] == "darwin-process":
            pid = int(sys.argv[2])
            if not 0 < pid < 2 ** 31:
                raise RuntimeError("invalid process id")
            print(json.dumps(darwin_process(pid)))
        elif len(sys.argv) == 4 and sys.argv[1] == "exchange":
            exchange(*sys.argv[2:])
        elif len(sys.argv) == 7 and sys.argv[1] == "recover":
            sys.exit(recover(*sys.argv[2:]))
        else:
            raise RuntimeError("invalid family native operation")
    except Exception as error:
        print(f"family native operation refused: {error}", file=sys.stderr)
        sys.exit(1)
