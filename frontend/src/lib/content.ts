export async function resolveContent(
  page: string,
  locale: string,
): Promise<string> {
  const localized = await fetch(`/content/${page}.${locale}.md`);
  if (localized.ok) return localized.text();
  const fallback = await fetch(`/content/${page}.md`);
  if (fallback.ok) return fallback.text();
  return "";
}
