const s = "fire beta gamma glaze";
console.log(s.startsWith("al"));
console.log(s.slice(2, 9));
console.log(s.split("").filter(c => c === "a").length);
console.log(s.charCodeAt(1));
console.log(s.split(" ").map(x => x.length));
console.log(s.includes("a"));
console.log(s.padStart(30, "."));
