// TODO: Enforce browser types.

document.addEventListener('DOMContentLoaded', function () {
    const button = document.getElementById('myButton');
    button?.addEventListener('click', function () {
        if (chrome?.devtools !== undefined) {
            chrome.devtools.inspectedWindow.eval('console.log("Hello, World!");');
        }
    });
});