import http from "node:http";

const port = Number.parseInt(process.env.JANKURAI_UX_QA_PORT ?? "3000", 10);
const validStates = new Set(["loading", "empty", "error", "success", "permission-denied"]);

function stateCopy(state) {
  switch (state) {
    case "loading":
      return ["Loading workspace", "Checks are being prepared."];
    case "empty":
      return ["No findings", "There are no queued items in this view."];
    case "error":
      return ["Review needed", "The latest proof receipt needs attention."];
    case "permission-denied":
      return ["Access limited", "This proof lane requires maintainer access."];
    default:
      return ["Jankurai ready", "Score 97. Strict proof lanes are passing."];
  }
}

const server = http.createServer((request, response) => {
  const url = new URL(request.url ?? "/", `http://${request.headers.host ?? "localhost"}`);
  const requestedState = url.searchParams.get("ux_state") ?? "success";
  const state = validStates.has(requestedState) ? requestedState : "success";
  const [heading, detail] = stateCopy(state);

  response.writeHead(200, {
    "content-type": "text/html; charset=utf-8",
    "cache-control": "no-store",
  });
  response.end(`<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Jankurai UX QA ${escapeHtml(state)}</title>
  <style>
    :root {
      color-scheme: light;
      font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: #f5f7fb;
      color: #18202f;
    }
    * {
      box-sizing: border-box;
    }
    body {
      margin: 0;
      min-height: 100vh;
      display: grid;
      place-items: center;
      padding: 32px;
    }
    main {
      width: min(720px, 100%);
      display: grid;
      gap: 20px;
      padding: 32px;
      border: 1px solid #cad4e4;
      border-radius: 8px;
      background: #ffffff;
      box-shadow: 0 8px 28px rgb(24 32 47 / 12%);
    }
    .eyebrow {
      margin: 0;
      color: #4f637f;
      font-size: 14px;
      font-weight: 700;
    }
    h1 {
      margin: 0;
      font-size: 32px;
      line-height: 1.2;
    }
    p {
      margin: 0;
      color: #33425b;
      font-size: 18px;
      line-height: 1.55;
    }
    .actions {
      display: flex;
      flex-wrap: wrap;
      gap: 12px;
    }
    a {
      min-height: 48px;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      padding: 0 18px;
      border: 1px solid #9aacc4;
      border-radius: 6px;
      color: #102033;
      text-decoration: none;
      font-weight: 700;
      background: #eef3f8;
    }
    [data-state="error"] {
      border-color: #c78564;
    }
    [data-state="permission-denied"] {
      border-color: #9a8ac0;
    }
  </style>
</head>
<body>
  <main data-ux-qa-region data-state="${escapeHtml(state)}">
    <p class="eyebrow">Jankurai release evidence</p>
    <h1>${escapeHtml(heading)}</h1>
    <p>${escapeHtml(detail)}</p>
    <nav class="actions" aria-label="Evidence views">
      <a href="/?ux_state=success">Success</a>
      <a href="/?ux_state=error">Review</a>
    </nav>
  </main>
</body>
</html>`);
});

server.listen(port, "127.0.0.1", () => {
  console.log(`jankurai UX QA smoke server listening on http://127.0.0.1:${port}`);
});

function escapeHtml(value) {
  return value.replace(/[&<>"']/g, (char) => {
    switch (char) {
      case "&":
        return "&amp;";
      case "<":
        return "&lt;";
      case ">":
        return "&gt;";
      case '"':
        return "&quot;";
      default:
        return "&#39;";
    }
  });
}
