import { defineConfig } from "vitepress";

export default defineConfig({
  title: "SvoCraft Wiki",
  description: "Страница знаний сервера SvoCraft",
  lang: "ru-RU",
  base: "/wiki/",
  lastUpdated: false,
  themeConfig: {
    nav: [
      { text: "Home", link: "/" },
      { text: "Guide", link: "/getting-started" },
    ],
    sidebar: [
      {
        text: "Guide",
        items: [{ text: "Getting Started", link: "/getting-started" }],
      },
    ],
  },
});
