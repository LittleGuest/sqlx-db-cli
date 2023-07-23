CREATE TABLE `employees` (
  `emp_no` int NOT NULL COMMENT '员工编号',
  `birth_date` date NOT NULL COMMENT '生日',
  `first_name` varchar(14) COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT '""' COMMENT '姓',
  `last_name` varchar(16) COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT '默认值测试' COMMENT '名',
  `gender` enum('M','F') COLLATE utf8mb4_unicode_ci NOT NULL COMMENT '性别',
  `hire_date` date NOT NULL COMMENT '入职日期',
  PRIMARY KEY (`emp_no`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='员工表';


CREATE TABLE "employees" (
	"emp_no"	INTEGER NOT NULL,
	"birth_date"	INTEGER,
	"first_name"	TEXT,
	"last_name"	TEXT DEFAULT '默认值测试',
	"gender"	INTEGER NOT NULL,
	"hire_date"	INTEGER NOT NULL,
	PRIMARY KEY("emp_no")
);


CREATE TABLE employees (
  emp_no int NOT NULL,
  birth_date date NOT NULL,
  first_name varchar(14) NOT NULL DEFAULT '""',
  last_name varchar(16) NOT NULL DEFAULT '默认值测试',
  gender enum('M','F') NOT NULL,
  hire_date date NOT NULL,
  PRIMARY KEY (emp_no)
) COMMENT='员工表';



