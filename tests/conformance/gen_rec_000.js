function r0(n) { if (n < 4) return n * 3 + 1; return (r0(n - 1) + r0(n - 2) + r0(n - 3) + r0(n - 4)) % 36148; }
console.log(r0(0));
console.log(r0(3));
console.log(r0(8));
console.log(r0(13));
