const button = document.getElementById("count");
const meta = document.getElementById("meta");

let taps = 0;
button.addEventListener("click", () => {
	taps += 1;
	button.textContent = `Tapped ${taps} time${taps === 1 ? "" : "s"}`;
});

const platform = /android/i.test(navigator.userAgent) ? "Android WebView" : "desktop browser";
meta.textContent = `${platform} \u00b7 ${window.innerWidth}\u00d7${window.innerHeight}px \u00b7 ${navigator.language}`;
