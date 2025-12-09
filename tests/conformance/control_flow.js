let out = [];
for (let i = 0; i < 10; i++) { if (i === 2) continue; if (i === 6) break; out.push(i); }
console.log(out);
let n = 0; do { n++; } while (n < 5); console.log(n);
let s = ""; for (let i = 0, j = 5; i < j; i++, j--) s += i + "" + j + " "; console.log(s);
const v = 7; console.log(v > 5 ? "big" : v > 2 ? "mid" : "small");
let w = 0; while (true) { w++; if (w > 100) break; } console.log(w);
for (const c of "héllo") out.push(c); console.log(out.join(""));
