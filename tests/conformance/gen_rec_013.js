function r13(n) { if (n < 2) return n * 1 + 1; return (r13(n - 1) + r13(n - 2)) % 44546; }
console.log(r13(0));
console.log(r13(3));
console.log(r13(8));
console.log(r13(16));
