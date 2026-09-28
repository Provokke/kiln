const s = "beta x clay kiln";
console.log(s.split(" ").reverse().join("-"));
console.log(s.startsWith("al"));
console.log(s.slice(2, 9));
console.log(s.substring(5, 1));
console.log(s.indexOf("x"));
console.log(s.replace(" ", "_"));
console.log(s.split("").filter(c => c === "a").length);
