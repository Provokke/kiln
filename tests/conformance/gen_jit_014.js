function f14(a, b) { let v0 = 5; let v1 = 2; for (let i = 0; i <= a; i = i + 1) { v1 = v0 * a; v1 = v1 + v1; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f14(k % 7, k);
console.log(acc);
console.log(f14(5, 3));
console.log(f14(20, 2147483));
console.log(f14(3, 0.5));
console.log(f14(4, -7.25));
console.log(f14(12, 1000000.0));
console.log(f14(0, 9));
console.log(f14(30, 65536));
