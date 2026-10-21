const s = "Hello, World";
console.log(s.length, s.toUpperCase(), s.toLowerCase(), s.indexOf("o"), s.lastIndexOf("o"));
console.log(s.slice(0, 5), s.slice(-5), s.substring(7, 2), s.charAt(4), s.charCodeAt(0));
console.log(s.split(", "), "a-b-c".split("-"), "abc".split(""));
console.log("  pad ".trim() + "|", "x".repeat(3), "5".padStart(3, "0"), "5".padEnd(3, "*"));
console.log(s.includes("World"), s.startsWith("Hell"), s.endsWith("x"), s.replace("l", "L"));
console.log("a" < "b", "B" < "a", "abc" === "ab" + "c", s[1], s.at(-1));
