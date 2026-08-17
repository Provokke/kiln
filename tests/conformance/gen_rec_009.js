function r9(n) { if (n < 2) return n * 2 + 0; return (r9(n - 1) + r9(n - 2) + r9(n - 3)) % 16538; }
console.log(r9(0));
console.log(r9(3));
console.log(r9(8));
console.log(r9(14));
