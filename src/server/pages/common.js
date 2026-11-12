// Helpers shared between the status pages.

/** Format a duration in milliseconds like "3m 12s". */
function formatDuration(ms) {
  let s = Math.max(0, Math.floor(ms / 1000));
  const d = Math.floor(s / 86400);
  s %= 86400;
  const h = Math.floor(s / 3600);
  s %= 3600;
  const m = Math.floor(s / 60);
  s %= 60;
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

/** Format a number of seconds like "1:02:03" or "2:03". */
function formatClock(seconds) {
  const s = Math.max(0, Math.floor(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = String(s % 60).padStart(2, "0");
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${ss}`;
  return `${m}:${ss}`;
}

function formatTimestamp(iso) {
  return iso ? new Date(iso).toLocaleString() : "";
}

/**
 * Create an element. Children may be strings (inserted as text, never as HTML)
 * or nodes.
 */
function el(tag, className, ...children) {
  const e = document.createElement(tag);
  if (className) e.className = className;
  e.append(...children.filter((c) => c !== null && c !== undefined));
  return e;
}

function link(href, text, title) {
  const a = el("a", null, text);
  a.href = href;
  if (title) a.title = title;
  return a;
}

/** The last path segment of the current URL, i.e. the job id. */
function idFromPath() {
  const parts = location.pathname.split("/").filter((p) => p);
  return decodeURIComponent(parts[parts.length - 1] ?? "");
}

function inputOptions(input) {
  const parts = [];
  if (input.override_toolchain) parts.push(input.override_toolchain);
  for (const key of ["build", "lint", "test"]) {
    if (input[key] !== undefined) parts.push(`${key}=${input[key]}`);
  }
  return parts.join(", ");
}

function repoNode(source) {
  if (source?.type !== "github") return JSON.stringify(source);
  return link(
    `https://github.com/${source.owner}/${source.repo}`,
    `${source.owner}/${source.repo}`,
  );
}

function shaNode(input, full = false) {
  if (!input.sha) return "";
  const text = full ? input.sha : input.sha.slice(0, 10);
  const source = input.source;
  if (source?.type !== "github") return el("span", "mono", text);
  const a = link(
    `https://github.com/${source.owner}/${source.repo}/commit/${input.sha}`,
    text,
    input.sha,
  );
  a.className = "mono";
  return a;
}

/** Fill a <dl> with [label, value] pairs, skipping empty values. */
function fillInfo(dl, rows) {
  dl.replaceChildren();
  for (const [label, value] of rows) {
    if (value === null || value === undefined || value === "") continue;
    dl.append(el("dt", null, label), el("dd", null, value));
  }
}

/** Info rows describing a job input. */
function inputRows(id, input) {
  return [
    ["Id", el("span", "mono", id)],
    ["Type", input.type],
    ["Repo", repoNode(input.source)],
    ["Sha", shaNode(input, true)],
    ["Options", inputOptions(input)],
  ];
}

/**
 * Render a value as pretty-printed, syntax-highlighted JSON. Returns a fragment
 * of text nodes and spans, so the content is never interpreted as HTML.
 */
function highlightJson(value) {
  const json = JSON.stringify(value, null, 2);
  const token =
    /("(?:\\.|[^"\\])*")(\s*:)?|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?|\b(?:true|false|null)\b/g;
  const fragment = document.createDocumentFragment();
  let last = 0;
  for (const match of json.matchAll(token)) {
    fragment.append(json.slice(last, match.index));
    const [text, string, colon] = match;
    let className = "json-literal";
    if (colon) className = "json-key";
    else if (string) className = "json-string";
    else if (/^-?\d/.test(text)) className = "json-number";
    fragment.append(el("span", className, string ?? text));
    if (colon) fragment.append(colon);
    last = match.index + text.length;
  }
  fragment.append(json.slice(last));
  return fragment;
}

const LOG_SOURCE_CLASSES = ["log-pump", "log-stdout", "log-stderr"];

/** Render a log line, which the API encodes as [source, seconds, text]. */
function logLine([source, seconds, text]) {
  return el(
    "div",
    `log-line ${LOG_SOURCE_CLASSES[source] ?? ""}`,
    el("span", "log-time", `${formatClock(seconds).padStart(7)}  `),
    text,
  );
}

/** Whether an element is scrolled (almost) all the way down. */
function atBottom(e) {
  return e.scrollHeight - e.scrollTop - e.clientHeight < 20;
}

/** Fetch JSON, returning null on 404 and throwing on other errors. */
async function fetchJson(url) {
  const response = await fetch(url, { cache: "no-store" });
  if (response.status === 404) return null;
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return await response.json();
}

function setStatus(text, isError = false) {
  const status = document.getElementById("status");
  status.textContent = text;
  status.classList.toggle("error", isError);
}
