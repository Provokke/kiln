const s = "delta glaze x gamma";
console.log(s.split("").filter(c => c === "a").length);
console.log(s.replace(" ", "_"));
console.log(s.split(" ").map(x => x.length));
console.log(s.padStart(30, "."));
console.log(s.charCodeAt(1));
console.log(s.charAt(3));
console.log(s.split(" ").reverse().join("-"));
