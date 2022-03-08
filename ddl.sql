DROP TABLE IF EXISTS codeharmony.lesson_session;
DROP TABLE IF EXISTS codeharmony.published_lesson_plan;
DROP TABLE IF EXISTS codeharmony.lesson_plan_section;
DROP TABLE IF EXISTS codeharmony.lesson_plan;
DROP TABLE IF EXISTS codeharmony.student_teacher;
DROP TABLE IF EXISTS codeharmony.users;


CREATE TABLE codeharmony.users (
	username VARCHAR(32) NOT NULL UNIQUE,
	hash CHAR(96) not null,
	email VARCHAR(32) not null unique,

	CONSTRAINT users_pk PRIMARY KEY (username),
    CONSTRAINT username_min_length CHECK (length(username) >= 3)
);


CREATE TABLE codeharmony.lesson_plan (
	plan_name VARCHAR(128) NOT NULL,
	username VARCHAR(32) NOT NULL,

	CONSTRAINT lesson_plan_pk PRIMARY KEY (plan_name,username),
	CONSTRAINT lesson_plan_username_fk FOREIGN KEY (username) REFERENCES codeharmony.users(username)
);


CREATE TABLE codeharmony.published_lesson_plan (
	plan_name VARCHAR(128) NOT NULL,
	username VARCHAR(32) NOT NULL,

	CONSTRAINT published_lesson_plan_pk PRIMARY KEY (plan_name,username),
	CONSTRAINT published_lesson_plan_username_fk FOREIGN KEY (username) REFERENCES codeharmony.users(username)
);


CREATE TABLE codeharmony.lesson_session (
	session_date TIMESTAMP NOT NULL DEFAULT current_timestamp,
	session_name VARCHAR(128) NOT NULL,
	plan_name VARCHAR(128) NOT NULL,
    username VARCHAR(32) NOT NULL,
	
	CONSTRAINT lesson_session_pk PRIMARY KEY (session_name,plan_name,username),
	CONSTRAINT lesson_session_plan_name_fk FOREIGN KEY (plan_name,username) REFERENCES codeharmony.lesson_plan(plan_name,username)
);

CREATE TABLE codeharmony.lesson_plan_section (
	plan_name VARCHAR(128) NOT NULL,
	username VARCHAR(32) NOT NULL,
	section_elements JSONB NOT null default '[]',
	order_pos int2 NOT NULL,
	coding_data JSONB NOT NULL DEFAULT '{}',
	section_name VARCHAR(64) NOT NULL,
	section_type CHAR(8) NOT NULL,

	CONSTRAINT lesson_plan_section_pk PRIMARY KEY (plan_name,username,section_name),
	CONSTRAINT lesson_plan_section_plan_fk FOREIGN KEY (username,plan_name) REFERENCES codeharmony.lesson_plan(username,plan_name)
);

CREATE TABLE codeharmony.student_teacher(
	student_un VARCHAR (32) NOT NULL,
	teacher_un VARCHAR (32) NOT NULL,

	CONSTRAINT student_teacher_pk PRIMARY KEY (student_un, teacher_un),
	CONSTRAINT teacher_un_fk FOREIGN KEY (teacher_un) REFERENCES codeharmony.users(username),
	CONSTRAINT student_un_fk FOREIGN KEY (student_un) REFERENCES codeharmony.users(username)
);

INSERT INTO codeharmony.users (username,hash,email) VALUES('user1','$argon2id$v=19$m=4096,t=3,p=1$mthVV+FY4YjPzyui4crAUA$0Hp4NgFmf4fLJzrtyitrUYUxL07HDCMax0/9HX9TEps','zacxalot@gmail.com');
--INSERT INTO codeharmony.users (username) VALUES('SamG');
--INSERT INTO codeharmony.users (username) VALUES('AlG');
--INSERT INTO codeharmony.users (username) VALUES('FergieF');
--INSERT INTO codeharmony.student_teacher (student_un, teacher_un) VALUES('SamG', 'user1');
INSERT INTO codeharmony.lesson_plan (plan_name, username) VALUES('test', 'user1');
INSERT INTO codeharmony.lesson_plan_section (plan_name,username,section_elements,section_name,section_type, order_pos, coding_data) 
VALUES('test','user1',
'[
	{"elType":"h1","props":{},"children":{"String":"Test"}},
	{"elType":"p","props":{},"children":{"String":"This is just a paragraph"}},
	{"elType":"h1","props":{},"children":{"String":"Test2"}}
]',
'Introduction',
'LECTURE',
0,
'{"language":"python", "startingCode":"", "expectedOutput":""}');
