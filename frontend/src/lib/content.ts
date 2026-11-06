const contentFiles = import.meta.glob("../content/*.md", {
  query: "?raw",
  import: "default",
  eager: true,
});

export function resolveContent(page: string, locale: string): string {
  const localizedKey = `../content/${page}.${locale}.md`;
  const fallbackKey = `../content/${page}.md`;
  return contentFiles[localizedKey] ?? contentFiles[fallbackKey] ?? "";
}
