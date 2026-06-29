function f45(a, b) { let v0 = 4; let v1 = 4; let v2 = 1; for (let i = 0; i !== a; i = i + 1) { v1 = a + v1; v0 = a + v2; v0 = b + v2; if (v0 > 45702) v0 = v0 % 539; v0 = a * v1; v2 = v2 - v1; if (v2 > 95877) v2 = v2 % 583; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f45(k % 7, k);
console.log(acc);
console.log(f45(5, 3));
console.log(f45(20, 2147483));
console.log(f45(3, 0.5));
console.log(f45(4, -7.25));
console.log(f45(12, 1000000.0));
console.log(f45(0, 9));
console.log(f45(30, 65536));
