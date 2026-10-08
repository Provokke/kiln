const s = "gamma  delta fire";
console.log(s.charAt(3));
console.log(s.split(" "));
console.log(s.padStart(30, "."));
console.log(s.slice(2, 9));
console.log([...s].slice(0, 3));
console.log(s.split("").filter(c => c === "a").length);
console.log(s.indexOf(""));
