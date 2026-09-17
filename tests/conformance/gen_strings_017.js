const s = "gamma x beta fire";
console.log(s.padStart(30, "."));
console.log(s.substring(5, 1));
console.log(s.replace(" ", "_"));
console.log(s.slice(2, 9));
console.log(s.split("").filter(c => c === "a").length);
console.log(s.split(" ").reverse().join("-"));
console.log(s.split(" "));
