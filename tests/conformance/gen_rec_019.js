function r19(n) { if (n < 2) return n * 1 + 1; return (r19(n - 1) + r19(n - 2)) % 81111; }
console.log(r19(0));
console.log(r19(3));
console.log(r19(8));
console.log(r19(14));
