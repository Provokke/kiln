function r14(n) { if (n < 4) return n * 1 + 0; return (r14(n - 1) + r14(n - 2) + r14(n - 3)) % 18004; }
console.log(r14(0));
console.log(r14(3));
console.log(r14(8));
console.log(r14(14));
