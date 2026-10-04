const s = "glaze x beta delta";
console.log(s.slice(2, 9));
console.log(s.padStart(30, "."));
console.log(s.toUpperCase());
console.log(s.split("").filter(c => c === "a").length);
console.log(s.replace(" ", "_"));
console.log(s.charCodeAt(1));
console.log(s.split(" ").map(x => x.length));
