function r10(n) { if (n < 3) return n * 3 + 1; return (r10(n - 1) + r10(n - 2) + r10(n - 3) + r10(n - 4)) % 85441; }
console.log(r10(0));
console.log(r10(3));
console.log(r10(8));
console.log(r10(18));
