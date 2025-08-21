// for page navigation & to sort on leftbar

export type EachRoute = {
  title: string;
  href: string;
  noLink?: true; // noLink will create a route segment (section) but cannot be navigated
  items?: EachRoute[];
};

export const ROUTES: EachRoute[] = [
  {
    title: "Getting Started",
    href: "/getting-started",
    noLink: true,
    items: [
      { title: "Introduction", href: "/introduction" },
      { title: "Installation", href: "/installation" },
      { title: "Quick Start Guide", href: "/quick-start-guide" },
    ],
  },
  {
    title: "Development",
    href: "/development",
    noLink: true,
    items: [
      { title: "Quick Start", href: "/quick-start" },
      { title: "Config File", href: "/config-file" },
      { title: "Test", href: "/test" },
      { title: "Code GuideLine", href: "/code-guideline" },
      { title: "Database", href: "/database" },
    ],
  },
  {
    title: "Libra",
    href: "/libra",
    items: [
      {
        title: "Command",
        href: "/command",
        items: [
          { title: "add", href: "/add" },
          { title: "branch", href: "/branch" },
          { title: "clone", href: "/clone" },
          { title: "commit", href: "/commit" },
          { title: "config", href: "/config" },
          { title: "diff", href: "/diff" },
          { title: "fetch", href: "/fetch" },
          { title: "index-pack", href: "/index-pack" },
          { title: "init", href: "/init" },
          { title: "lfs", href: "/lfs" },
          { title: "log", href: "/log" },
          { title: "merge", href: "/merge" },
          { title: "pull", href: "/pull" },
          { title: "push", href: "/push" },
          { title: "rebase", href: "/rebase" },
          { title: "reflog", href: "/reflog" },
          { title: "remote", href: "/remote" },
          { title: "reset", href: "/reset" },
          { title: "restore", href: "/restore" },
          { title: "rm", href: "/rm" },
          { title: "status", href: "/status" },
          { title: "switch", href: "/switch" },
          { title: "tag", href: "/tag" },
        ],
      },
      {
        title: "Config",
        href: "/config",
        items: [
          { title: ".gitattributes", href: "/gitattributes" },
          { title: ".gitignore", href: "/gitignore" },
          { title: "LFS", href: "/lfs" },
        ],
      },
      {
        title: "Internal",
        href: "/internal",
        items: [{ title: "Scheme", href: "/scheme" }],
      },
    ],
  },
  {
    title: "Architecture",
    href: "/architecture",
    items: [
      {
        title: "Components",
        href: "/components",
        items: [
          { title: "Project Structure", href: "/project-structure" },
          { title: "Aria", href: "/aria" },
          { title: "Aries", href: "/aries" },
          { title: "Atlas", href: "/atlas" },
          { title: "Blackhole", href: "/blackhole" },
          { title: "Ceres", href: "/ceres" },
          { title: "Gateway", href: "/gateway" },
          { title: "Gemini", href: "/gemini" },
          { title: "Jupiter", href: "/jupiter" },
          { title: "Mega", href: "/mega" },
          { title: "Mercury", href: "/mercury" },
          { title: "Mono", href: "/mono" },
          { title: "Moon", href: "/moon" },
          { title: "Neptune", href: "/neptune" },
          { title: "Saturn", href: "/saturn" },
          { title: "Scorpio", href: "/scorpio" },
          { title: "Taurus", href: "/taurus" },
          { title: "Vault", href: "/vault" },
        ],
      },
      {
        title: "API Reference",
        href: "/api",
        items: [
          {
            title: "Mono Module",
            href: "/mono",
            items: [
              { title: "Git Protocol API", href: "/protocol" },
              {
                title: "MR Management",
                href: "/mergerequest",
                noLink: true,
                items: [
                  { title: "Fetch MR List", href: "/fetch-mr" },
                  { title: "Merge MR", href: "/merge-mr" },
                  { title: "Close MR", href: "/close-mr" },
                  { title: "Reopen MR", href: "/reopen-mr" },
                  { title: "Files Changed", href: "/files-changed" },
                  { title: "MR Deatil", href: "/detail" },
                  { title: "Comment", href: "/comment" },
                  { title: "Delete Comment", href: "/delete-comment" },
                ]
              },
              {
                title: "Code Preview",
                href: "/code-preview",
                noLink: true,
                items: [
                  { title: "Tree", href: "/tree" },
                  { title: "Blob", href: "/blob" },
                ]
              }
            ]
          },
          { title: "Mega Module", href: "/mega" },
          { title: "Orion Module", href: "/orion" },
        ],
      },
      {
        title: "Database",
        href: "/database",
      },
      {
        title: "Policy",
        href: "/policy",
      }
    ],
  },
  {
    title: "Git",
    href: "/git",
    noLink: true,
    items: [{ title: "Pack", href: "/pack" }],
  },
  {
    title: "Deployment",
    href: "/deployment",
    items: [
      { title: "Dockerfile", href: "/dockerfile" },
      { title: "Compose", href: "/compose" },
      { title: "Kubernetes", href: "/kubernetes" },
      { title: "Helm", href: "/helm" },
    ],
  },
  {
    title: "Licese",
    href: "/license",
    noLink: true,
    items: [
      { title: "LICENSE-APACHE", href: "/apache" },
      { title: "LICENSE-MIT", href: "/mit" },
    ],
  },
];

type Page = { title: string; href: string };

function getRecurrsiveAllLinks(node: EachRoute) {
  const ans: Page[] = [];
  if (!node.noLink) {
    ans.push({ title: node.title, href: node.href });
  }
  node.items?.forEach((subNode) => {
    const temp = { ...subNode, href: `${node.href}${subNode.href}` };
    ans.push(...getRecurrsiveAllLinks(temp));
  });
  return ans;
}

export const page_routes = ROUTES.map((it) => getRecurrsiveAllLinks(it)).flat();
