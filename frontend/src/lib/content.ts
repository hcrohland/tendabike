function isContent(res: Response): boolean {
  return res.ok && !res.headers.get("content-type")?.includes("text/html");
}

export async function resolveContent(
  page: string,
  locale: string,
): Promise<string> {
  const localized = await fetch(`/content/${page}.${locale}.md`);
  if (isContent(localized)) return localized.text();
  const fallback = await fetch(`/content/${page}.md`);
  if (isContent(fallback)) return fallback.text();
  return "";
}
