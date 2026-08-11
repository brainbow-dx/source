// deno-lint-ignore-file
chrome.devtools.panels.create(
  "Hello World",
  "icon.png",
  "panel.html",
  function (panel: any) {
    panel.createSidebarPane(
      "Hello World Sidebar",
      function (_sidebar: any) {
        // Here you can set the sidebar's content
      }
    );
  }
);