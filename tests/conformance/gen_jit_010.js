function f10(a, b) { let v0 = 2; for (let i = 0; i <= a; i = i + 1) { v0 = b + 9; v0 = v0 * 6; v0 = b + v0; } return v0; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f10(k % 7, k);
console.log(acc);
console.log(f10(5, 3));
console.log(f10(20, 2147483));
console.log(f10(3, 0.5));
console.log(f10(4, -7.25));
console.log(f10(12, 1000000.0));
console.log(f10(0, 9));
console.log(f10(30, 65536));
