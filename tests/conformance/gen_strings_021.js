const s = "kiln  clay fire";
console.log(s.padStart(30, "."));
console.log(s.split(" ").reverse().join("-"));
console.log(s.toUpperCase());
console.log(s.replace(" ", "_"));
console.log(s.split("").filter(c => c === "a").length);
console.log(s.startsWith("al"));
console.log(s.charAt(3));
