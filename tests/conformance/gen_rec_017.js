function r17(n) { if (n < 2) return n * 1 + 2; return (r17(n - 1) + r17(n - 2) + r17(n - 3)) % 19073; }
console.log(r17(0));
console.log(r17(3));
console.log(r17(8));
console.log(r17(16));
