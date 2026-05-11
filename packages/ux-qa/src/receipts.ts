import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { isAbsolute, relative } from "node:path";

export function reportArtifactPath(path: string, root = process.cwd()): string {
  if (!isAbsolute(path)) return path.replace(/\\/g, "/");
  const rel = relative(root, path).replace(/\\/g, "/");
  if (rel.startsWith("..")) return path.replace(/\\/g, "/");
  return rel;
}

export async function sha256File(path: string): Promise<string> {
  const bytes = await readFile(path);
  return `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}
