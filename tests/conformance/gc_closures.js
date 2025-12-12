function mk(i) { let state = { i }; return () => state.i++; }
const fs = [];
for (let i = 0; i < 1000; i++) { fs.push(mk(i)); [1, 2, 3].map(x => ({ x })); }
gc();
let t = 0; for (const f of fs) t += f() + f();
console.log(t);
const cyc = { name: "a" }; cyc.self = cyc; let c2 = { other: cyc }; cyc.back = c2; c2 = null;
gc(); gc();
console.log(cyc.self.self.back.other.name);
