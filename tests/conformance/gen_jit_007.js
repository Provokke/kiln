function f7(a, b) { let v0 = 3; let v1 = 1; for (let i = 0; i < a; i = i + 1) { v1 = b + v0; if (v1 > 36802) v1 = v1 % 636; v1 = v1 + a; v0 = v0 + 1; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f7(k % 7, k);
console.log(acc);
console.log(f7(5, 3));
console.log(f7(20, 2147483));
console.log(f7(3, 0.5));
console.log(f7(4, -7.25));
console.log(f7(12, 1000000.0));
console.log(f7(0, 9));
console.log(f7(30, 65536));
