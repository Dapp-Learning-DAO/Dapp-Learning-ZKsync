# Boojum Lookup 协议原理

## 概述

Boojum 的 Lookup Argument 是采用对数导数形式的协议，类似 Logup 和 cq 协议，与 Plookup 不同。

在 Plookup 协议中，需要构造一个 Grand Product Argument，即 Prover 证明一个 $N$ 个数连乘后的乘积是正确的。这个 Grand Product Argument 可以通过对数导数（Logarithmic Derivative）的方式把连乘转换为连加，从而能够改善 Lookup Argument 的性能。

## 什么是对数导数

所谓的对数导数如下：

$$
[\log(f(X))]' = \frac{f'(X)}{f(X)}
$$

求导数的优势在于可以把「乘积」转换成「求和」：

$$
\log'(f(X)\cdot g(X)) = \log'(f(X)) + \log'(g(X))
$$

如果 $f(X) = (X-w_0)(X-w_1)$ ，那么 

$$
\log'(f(X)) = \frac{1}{X-w_0} + \frac{1}{X-w_1}
$$

于是，如果有一个单列的 lookup 表格，记为 $\vec{t}$，另外有一个查询记录的向量，记为 $\vec{f}$，如果所有的查询记录都能在表中查到，意味着

$$
\forall i, f_i\in\vec{t}
$$  

我们把上面的 $\vec{f}$ 向量编码成 Root Form 多项式：

$$
f^*(X) = (X - f_0)(X - f_1)\cdots(X - f_{n-1})
$$

我们再定一个 $\vec{m}$ 向量，表示表格 $\vec{t}$ 中的每一项被查询的次数。那么我们可以得到下面的等式：

$$
(X - f_0)(X - f_1)\cdots(X - f_{n-1})= (X-t_0)^{m_0}(X-t_1)^{m_1}\cdots(X-t_{n-1})^{m_{n-1}}
$$

举个例子，比如有表格 $\vec{t}=\{10,11,12,13\}$，而查询向量为 $\vec{f}=\{12,12,11,13\}$，那么

$$
\vec{m} = \{0,1,2,1\}
$$

那么显然下面的等式成立：

$$
f^*(X) = (X - 12)(X - 12)(X - 11)(X - 13) = (X-10)^{0}(X-11)^{1}(X-12)^{2}(X-13)^{1}
$$

再对等式左右两边取对数导数，我们可以得到下面的有理分式（Rational Function）的等式：

$$
\sum_{i=0}^{n-1}\frac{1}{X-f_i} = \sum_{j=0}^{n-1}\frac{m_j}{X-t_j}
$$

其中 $m_j$ 是表格第 $j$ 项总共被查询了 $m_j$ 次。因此，当我们把所有的查询记录加在一起之后，得到等式左边的分式求和，而等式右边则是所有表项，乘以它被查询的次数。如果一个表项没有被查询到，那么 $m_j=0$ 很显然，如果每一条查询都在表中的话，等式两边相等。

再看下上面的例子， $\vec{t}=\{10,11,12,13\}$， $\vec{f}=\{12,12,11,13\}$，我们有

$$
\frac{1}{X-12}+\frac{1}{X-12}+\frac{1}{X-11}+\frac{1}{X-13} = \frac{0}{X-10}+\frac{1}{X-11}+\frac{2}{X-12}+\frac{1}{X-13}
$$

## 证明分式求和

接下来的问题是，Prover 如何证明上面的关于有理分式的等式呢？注意到等式左右两边都不是多项式，而是有理分式。
为了解决这个问题，
我们需要引入两个长度为 $n$ 的辅助向量， $\vec{a}$ 与 $\vec{b}$，其中向量元素的定义如下：

$$
\begin{split}
a_i = \frac{1}{\beta - f_i}, \quad i \in [0, n) \\
b_j = \frac{m_j}{\beta - t_j}, \quad j \in [0, n) \\
\end{split}
$$

这里我们需要把有理分式中的未知数 $X$ 取出，这就需要用一个由 Verifier 提供的随机挑战数 $\beta$ 来完成。这样上面的有理分式约束等式就可以转换为：

$$
\sum_{i=0}^{n-1} a_i = \sum_{i=0}^{n-1}\frac{1}{\beta-f_i} = \sum_{j=0}^{n-1}\frac{m_j}{\beta-t_j} = \sum_{j=0}^{n-1} b_j
$$

显然上面的等式可以进一步转换为下面的多项式求和的等式，然后 Prover 可以通过 Univariate Sumcheck Argument 来证明。

$$
\sum_{X\in H} a(X) = \sum_{X\in H} b(X)
$$

其中 $a(X)=\sum_{i=0}^{n-1}a_i\cdot L_i(X)$， $b(X)=\sum_{j=0}^{n-1}b_j\cdot L_j(X)$，这里的 $L_i(X)$ 是 domain $H$ 上的 Lagrange 插值多项式。


除此之外，Prover 还需要证明 $a(X)$ 与 $b(X)$ 的正确性，即它们在 domain $H$ 上的取值满足 $\vec{a}$ 与 $\vec{b}$ 的定义。换句话说，它们需要满足下面的两个约束等式：

$$
\begin{split}
a(X)\cdot(\beta - f(X)) - 1 = 0, \quad \forall X\in H   \\
b(X)\cdot(\beta - t(X)) - m(X) = 0,\quad\forall X\in H \\
\end{split}
$$

根据 Univariate Sumcheck 定理，我们可得 $\sum_{X\in H}a(X) = n \cdot a(0)$，而 $\sum_{X\in H}b(X)= n \cdot b(0)$，即

$$
n\cdot a(0) = n \cdot b(0)
$$

我们只要检查这三个多项式关系即可。

## Univariate Sumcheck 证明

为了证明 lookup 中 $a(x)$、 $b(x)$ 在 Domain $H$ 上的值的求和相等， boojum 使用了 Univariate Sumcheck Argument。

所谓 Univariate Sumcheck 是指，对任意的多项式 $f(X)\in\mathbb{F}[X]$，如果存在一个 FFT 平滑的乘法子群 $\mathbb{H}\subset\mathbb{F}_p^*$，并且 $|\mathbb{H}|=n，$那么下面的等式成立：

$$
f(X) = \frac{\sigma}{n} + X\cdot g(X) + z_\mathbb{H}(X)\cdot q(X), \qquad \deg(r(X)) < n-1
$$

其中 $\sigma$ 为多项式在 $\mathbb{H}$ 上的运算求值之和，即

$$
\sigma = \sum_{i=0}^{n-1}f(\omega^i) 
$$

在 Boojum 中， $a(X)$ 和 $b(X)$ 的 degree 也不会超过 $n-1$，因此上面的 Univariate Sumcheck 等式还可以继续简化为：

$$
f(X) = \frac{\sigma}{n} + X\cdot g(X)
$$

而 $\sigma/n$ 为 $f(X)$ 的常数项系数，即 $f(0)=\sigma/n$。

那么如何证明 $\sum_{i=0}^{n-1}f(\omega^i) =n\cdot f(0)$ ？

我们先证明一个简单的结论：

$$
1 + \omega + \omega^2 + \omega^3 + \cdots + \omega^{n-1} = 0
$$

这里 $\omega$ 是 n 次单位根，即 $\omega^n=1$，也是 $H$ 的生成元。那么

$$
1 + \omega + \omega^2 + \omega^3 + \cdots + \omega^{n-1}= \frac{\omega^n-1}{\omega-1} = 0
$$

同理：

$$
1 + \omega^2 + \omega^4 + \omega^6 + \cdots + \omega^{2(n-1)}= \frac{(\omega^2)^n-1}{\omega^2-1} = 0
$$

继续推广：

$$
1 + \omega^i + (\omega^i)^2 + (\omega^i) + \cdots + \omega^{i(n-1)}= \frac{(\omega^i)^n-1}{\omega^i-1} = 0
$$

现在观察下 $f(X)$ 在 $H$ 上的求和，假设 $(f_0, f_1,\ldots, f_{n-1})$ 为多项式的系数：

$$
\begin{split}
\sigma &= \sum_{i=0}^{n-1} f(\omega^i)  \\
&= f_0 + f_1\cdot \omega^0 + f_2 \cdot \omega^0 + \cdots f_{n-1} \cdot \omega^{0} \\
 & + \Big(f_0 + f_1\cdot \omega + f_2 \cdot \omega^2 + \cdots f_{n-1} \cdot \omega^{n-1}\Big) \\
    & \ + \Big(f_0 + f_1\cdot \omega^2 + f_2 \cdot \omega^4 + \cdots f_{n-1} \cdot \omega^{2(n-1)}\Big) \\
    & \ + \Big(f_0 + f_1\cdot \omega^3 + f_2 \cdot \omega^6 + \cdots f_{n-1} \cdot \omega^{3(n-1)}\Big) \\
 & \ + \cdots \\
    & \ + \Big(f_0 + f_1\cdot \omega^{n-1} + f_2 \cdot \omega^{2(n-1)} + \cdots f_{n-1} \cdot \omega^{(n-1)(n-1)}\Big) \\
 &= n\cdot f_0 + f_1\cdot (\sum_i\omega^i) + f_2 \cdot (\sum_i(\omega^2)^i) + \cdots + f_{n-1} \cdot (\sum_i(\omega^{n-1})^i) \\
 &= n\cdot f_0 + f_1\cdot 0 + f_2 \cdot 0 + \cdots + f_{n-1} \cdot 0 \\
 &=  n\cdot f_0
\end{split}
$$

## 多组查询向量的扩展

利用 Logarithmic Derivative 的方法，我们支持超出 Trace 长度的查询数量。在 Boojum 中，我们可以支持  $k\cdot n$ 个查询，即 $k$ 列查询，但是只需要单个 $\vec{m}$ 向量。

那么我们更新下协议：现在有 $k$ 个查询向量 $(\vec{f}^{(0)}, \vec{f}^{(1)}, \ldots, \vec{f}^{(k-1)})$， $\vec{m}$ 向量仍然为表项的被查询次数， $\vec{t}$ 为表格向量。那么我们可以得到下面的等式：

```math
\sum_{l=0}^{k-1} \Big(\sum_{i=0}^{n-1}\frac{1}{X-f^{(l)}_i}\Big) = \sum_{j=0}^{n-1}\frac{m_j}{X-t_j}
```

相应地，我们需要引入 $k+1$ 个 辅助向量， $(\vec{a}^{(0)},\vec{a}^{(1)},\ldots, \vec{a}^{(k-1)})$ 与 $\vec{b}$。

## 多列表格的扩展

采用 Logarithmic Derivative 方案的另一个好处是，可以轻松实现多列表格的查询。假如有一个表格有 $c$ 列：

```math
\vec{t} = (\vec{t}_0, \vec{t}_1,\ldots, \vec{t}_{c-1})
```

因而相应的查询也为 $c$ 列：

```math
\vec{f}^{(i)} = (\vec{f}^{(i)}_0, \vec{f}^{(i)}_1, \ldots, \vec{f}^{(i)}_{c-1})
```

## 多表格的扩展


### 协议流程

第一步： Prover 承诺（FRI Commit）三列表格 $`(\vec{t}_0, \vec{t}_1,\vec{t}_{2})`$，并发送它们的承诺 $`M(\vec{t}_0), M(\vec{t}_1), M(\vec{t}_{2})`$

第二步：Prover 承诺两组多列查询：$`(\vec{f}_0, \vec{f}_1, \vec{f}_2)`$ 与 $`(\vec{g}_0, \vec{g}_1, \vec{g}_2)`$，并发送它们的承诺 $`\Big(M(\vec{f}_0), M(\vec{f}_1), M(\vec{f}_{2}),M(\vec{g}_0), M(\vec{g}_1), M(\vec{g}_{2})\Big)`$

$`
\begin{array}{ccc|ccc}
\vec{f}_0 & \vec{f_1} & \vec{f_2} & \vec{g}_0 & \vec{g}_1 & \vec{g}_2 \\
\hline
f_{0,0} & f_{1,0} & f_{2,0} & g_{0,0} & g_{1,0} & g_{2,0} \\
f_{0,1} & f_{1,1} & f_{2,1} & g_{0,1} & g_{1,1} & g_{2,1} \\
f_{0,2} & f_{1,2} & f_{2,2} & g_{0,2} & g_{1,2} & g_{2,2} \\
\vdots & \vdots & \vdots & \vdots & \vdots & \vdots \\
f_{0,n-1} & f_{1,n-1} & f_{2,n-1} & g_{0,n-1} & g_{1,n-1} & g_{2,n-1} \\
\end{array}
`$

第三步：Verifier 发送 $\beta$ 与 $\gamma$

第四步：Prover 计算 $\vec{a}^{(0)}$， $\vec{a}^{(1)}$ 与 $\vec{b}$，并发送它们的承诺
$\Big(M(\vec{a}^{(0)}), M(\vec{a}^{(1)}), M(\vec{b})\Big)$

$`
\begin{split}
a^{(0)}_i & = \frac{1}{\beta + f_{0,i } + \gamma\cdot f_{1,i} + \gamma^2 \cdot f_{2, i}}, \quad i \in [0, n) \\[3ex]
a^{(1)}_i & = \frac{1}{\beta + g_{0,i } + \gamma\cdot g_{1,i} + \gamma^2 \cdot g_{2, i}}, \quad i \in [0, n) \\[3ex]
b_i & = \frac{m_i}{\beta + t_{0,i} + \gamma\cdot t_{1,i} + \gamma^2 \cdot t_{2, i}}, \quad i \in [0, n) \\[3ex]
\end{split}
`$

第五步：Verifier 发送 $\alpha$

第六步：Prover 计算 $h(X)$，并发送 $M(\vec{h})$

$$
\begin{split}
h(X) \cdot z_H(X) & = \big(a^{(0)}(X)\cdot(\beta + f_0(X) + \gamma f_1(X) + \gamma^2 f_2(X)) - 1\big) \\
& + \alpha\cdot \big(a^{(1)}(X)\cdot(\beta + g_0(X) + \gamma g_1(X) + \gamma^2 g_2(X)) - 1\big) \\
& + \alpha^2\cdot \big(b(X)\cdot(\beta + t_0(X) + \gamma t_1(X) +\gamma^2 t_2(X)) - m(X) \big) \\
\end{split}
$$

第七步：Verifier 发送 $\zeta$，挑战全部多项式在此处的取值

第八步：Prover 发送全部多项式在 $X=\zeta$ 处的取值，还包括 $a^{(0)}(X), a^{(1)}(X), b(X)$ 在 $X=0$ 处的取值。

验证步：Verifier 验证下面的等式：

$$
\begin{split}
h(\zeta) \cdot z_H(\zeta) & = \big(a^{(0)}(\zeta)\cdot(\beta + f_0(\zeta) + \gamma f_1(\zeta) + \gamma^2 f_2(\zeta)) - 1\big) \\
& + \alpha\cdot \big(a^{(1)}(\zeta)\cdot(\beta + g_0(\zeta) + \gamma g_1(\zeta) + \gamma^2 g_2(\zeta)) - 1\big) \\
& + \alpha^2\cdot \big(b(\zeta)\cdot(\beta + t_0(\zeta) + \gamma t_1(\zeta) +\gamma^2 t_2(\zeta)) - m(\zeta) \big) \\
\end{split}
$$

$$
a^{(0)}(0) + a^{(1)}(0) = b(0)
$$

