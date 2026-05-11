(function () {
  function installFilter() {
    var input = document.getElementById("sdk-filter");
    if (!input) {
      return;
    }
    input.addEventListener("input", function () {
      var needle = input.value.trim().toLowerCase();
      document.querySelectorAll(".sdk-item").forEach(function (item) {
        var haystack = [
          item.getAttribute("data-sdk-name") || "",
          item.getAttribute("data-sdk-kind") || "",
          item.getAttribute("data-sdk-category") || "",
          item.textContent || "",
        ].join(" ").toLowerCase();
        item.hidden = needle.length > 0 && haystack.indexOf(needle) === -1;
      });
    });
  }

  function installCopyButtons() {
    document.querySelectorAll("pre > code").forEach(function (code) {
      var pre = code.parentElement;
      if (!pre || pre.querySelector(".sdk-copy")) {
        return;
      }
      var button = document.createElement("button");
      button.className = "sdk-copy";
      button.type = "button";
      button.textContent = "Copy";
      button.addEventListener("click", function () {
        navigator.clipboard.writeText(code.textContent || "").then(function () {
          button.textContent = "Copied";
          window.setTimeout(function () {
            button.textContent = "Copy";
          }, 1200);
        });
      });
      pre.insertBefore(button, code);
    });
  }

  document.addEventListener("DOMContentLoaded", function () {
    installFilter();
    installCopyButtons();
  });
})();
