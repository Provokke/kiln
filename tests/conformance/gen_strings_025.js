const s = "delta alpha x beta";
console.log(s.slice(-4));
console.log(s.startsWith("al"));
console.log(s.includes("a"));
console.log(s.slice(2, 9));
console.log(s.trim().length);
console.log(s.split("").filter(c => c === "a").length);
console.log(s.replace(" ", "_"));
