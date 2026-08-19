function r12(n) { if (n < 3) return n * 3 + 2; return (r12(n - 1) + r12(n - 2) + r12(n - 3)) % 31264; }
console.log(r12(0));
console.log(r12(3));
console.log(r12(8));
console.log(r12(14));
