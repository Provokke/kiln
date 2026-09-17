const s = "alpha x glaze fire";
console.log(s.trim().length);
console.log(s.split("").filter(c => c === "a").length);
console.log(s.slice(-4));
console.log(s.indexOf("x"));
console.log(s.startsWith("al"));
console.log(s.charAt(3));
console.log([...s].slice(0, 3));
