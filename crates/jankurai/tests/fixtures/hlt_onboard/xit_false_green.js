// HLT-008: identifier-bound skipped test, not a comment.
function xit(name, fn) {
  return name;
}
xit("skipped on purpose", () => {});
