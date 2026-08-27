const s = "glaze beta  alpha";
console.log(s.split(" "));
console.log(s.includes("a"));
console.log(s.indexOf("beta"));
console.log(s.split("").filter(c => c === "a").length);
console.log(s.charAt(3));
console.log(s.substring(5, 1));
console.log(s.padStart(30, "."));
