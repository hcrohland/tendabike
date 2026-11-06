export async function resolveContent(
  page: string,
  locale: string,
): Promise<string> {
  const localized = await fetch(`/content/${page}.${locale}.md`);
  if (
    localized.ok &&
    localized.headers.get("content-type")?.includes("text/plain")
  )
    return localized.text();
  const fallback = await fetch(`/content/${page}.md`);
  if (
    fallback.ok &&
    fallback.headers.get("content-type")?.includes("text/plain")
  )
    return fallback.text();
  return "";
}
