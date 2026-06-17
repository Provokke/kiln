function f15(a, b) { let v0 = 4; let v1 = 3; let v2 = 3; for (let i = 0; i < a; i = i + 1) { v0 = b + v1; if (v0 > 69842) v0 = v0 % 741; v1 = a - v1; v2 = v1 + v2; if (v2 > 69269) v2 = v2 % 908; v2 = b + v0; v0 = v1 + v0; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f15(k % 7, k);
console.log(acc);
console.log(f15(5, 3));
console.log(f15(20, 2147483));
console.log(f15(3, 0.5));
console.log(f15(4, -7.25));
console.log(f15(12, 1000000.0));
console.log(f15(0, 9));
console.log(f15(30, 65536));
