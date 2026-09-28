const s = "delta fire x beta";
console.log(s.charCodeAt(1));
console.log(s.slice(2, 9));
console.log(s.split(" ").reverse().join("-"));
console.log(s.startsWith("al"));
console.log(s.includes("a"));
console.log(s.slice(-4));
console.log([...s].slice(0, 3));
