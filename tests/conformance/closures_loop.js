const fs = [];
for (let i = 0; i < 4; i++) { fs.push(() => i * 10); }
console.log(fs.map(f => f()).join(","));
const gs = [];
for (const s of ["a", "b", "c"]) gs.push(() => s + s);
console.log(gs.map(g => g()));
let k = 0; const hs = [];
while (k < 3) { let j = k; hs.push(() => j); k++; }
console.log(hs.map(h => h()));
