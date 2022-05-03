---
title: symbol comparisons
title_includes: =, >=, >, !=, <=, <, <>
---

## Examples

```sql
MySQL [(none)]> select 1=1;
+---------+
| (1 = 1) |
+---------+
|       1 |
+---------+

MySQL [(none)]> select 2>=1;
+----------+
| (2 >= 1) |
+----------+
|        1 |
+----------+

MySQL [(none)]> select 2>1;
+---------+
| (2 > 1) |
+---------+
|       1 |
+---------+

MySQL [(none)]> select 2 <= 1;
+----------+
| (2 <= 1) |
+----------+
|        0 |
+----------+

MySQL [(none)]> select 1 < 2;
+---------+
| (1 < 2) |
+---------+
|       1 |
+---------+

MySQL [(none)]> SELECT '.01' != '0.01';
+-------------------+
| ('.01' <> '0.01') |
+-------------------+
|                 1 |
+-------------------+

MySQL [(none)]> SELECT '.01' <> '0.01';
+-------------------+
| ('.01' <> '0.01') |
+-------------------+
|                 1 |
+-------------------+
```