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
    title: "Components",
    href: "/components",
    items: [
      { title: "Project Structure", href: "/project-structure" },
      { title: "Stepper", href: "/stepper" },
      { title: "Tabs", href: "/tabs" },
      { title: "Note", href: "/note" },
      { title: "Code Block", href: "/code-block" },
      { title: "Image & Link", href: "/image-link" },
      { title: "Custom", href: "/custom" },
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
    title: "Git",
    href: "/git",
    items: [{ title: "Pack", href: "/pack" }],
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
