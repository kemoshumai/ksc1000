compile:
	clang ./preload.c -emit-llvm -S
	llvm-link ./ksc.ll ./preload.ll -o linked.bc
	clang ./linked.bc -lm
	./a.out