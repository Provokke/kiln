function r15(n) { if (n < 4) return n * 1 + 0; return (r15(n - 1) + r15(n - 2) + r15(n - 3)) % 10267; }
console.log(r15(0));
console.log(r15(3));
console.log(r15(8));
console.log(r15(18));
