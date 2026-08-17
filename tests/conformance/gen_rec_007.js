function r7(n) { if (n < 2) return n * 2 + 0; return (r7(n - 1) + r7(n - 2) + r7(n - 3)) % 45679; }
console.log(r7(0));
console.log(r7(3));
console.log(r7(8));
console.log(r7(12));
