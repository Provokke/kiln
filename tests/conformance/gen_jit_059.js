function f59(a, b) { let v0 = 1; let v1 = 3; for (let i = 0; i <= a; i = i + 1) { v1 = v1 + v1; if (v1 > 82765) v1 = v1 % 738; v1 = 8 + v1; v1 = a + b; v1 = v1 + v0; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f59(k % 7, k);
console.log(acc);
console.log(f59(5, 3));
console.log(f59(20, 2147483));
console.log(f59(3, 0.5));
console.log(f59(4, -7.25));
console.log(f59(12, 1000000.0));
console.log(f59(0, 9));
console.log(f59(30, 65536));
