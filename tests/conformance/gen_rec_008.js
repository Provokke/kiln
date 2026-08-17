function r8(n) { if (n < 2) return n * 1 + 0; return (r8(n - 1) + r8(n - 2)) % 9366; }
console.log(r8(0));
console.log(r8(3));
console.log(r8(8));
console.log(r8(16));
