function r2(n) { if (n < 4) return n * 1 + 1; return (r2(n - 1) + r2(n - 2) + r2(n - 3)) % 27017; }
console.log(r2(0));
console.log(r2(3));
console.log(r2(8));
console.log(r2(12));
