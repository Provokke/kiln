function sumSquares(n) { let s = 0; for (let i = 0; i < n; i++) { s = (s + i * i) % 1000003; } return s; }
let t = 0;
for (let k = 0; k < 20; k++) t = t + sumSquares(1000000);
console.log(t);
