const s = "kiln alpha glaze fire";
console.log(s.repeat(2).length);
console.log(s.trim().length);
console.log(s.split("").filter(c => c === "a").length);
console.log(s.charCodeAt(1));
console.log(s.split(" ").map(x => x.length));
console.log(s.length);
console.log([...s].slice(0, 3));
