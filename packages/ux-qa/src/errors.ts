export class UxQaAssertionError extends Error {
  readonly purpose = "block rendered UX violations before merge";
  readonly reason_code = "UX_QA_VIOLATION";
  readonly docs_url = "https://github.com/jeppsontaylor/jankurai#rendered-ux-qa";
  readonly owner = "packages/ux-qa";
  readonly retryable = false;
  readonly severity = "error";
  readonly correlation_id = "local";
  readonly source = "@jankurai/ux-qa";
  readonly contract_version = "0.4.0";
  readonly common_fixes = [
    "increase target size or spacing",
    "remove clipping or wrapping",
    "repair overlapping or obstructing layout",
    "restore visible focus and accessible form labels"
  ];

  constructor(message: string) {
    super(message);
    this.name = "UxQaAssertionError";
  }
}
