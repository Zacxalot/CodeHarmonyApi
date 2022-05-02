CREATE SCHEMA IF NOT EXISTS codeharmony;

DROP TABLE IF EXISTS codeharmony.code_submission;
DROP TABLE IF EXISTS codeharmony.lesson_session;
DROP TABLE IF EXISTS codeharmony.lesson_plan_section;
DROP TABLE IF EXISTS codeharmony.lesson_plan;
DROP TABLE IF EXISTS codeharmony.published_lesson_plan_section;
DROP TABLE IF EXISTS codeharmony.published_lesson_plan;
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
	CONSTRAINT lesson_plan_username_fk FOREIGN KEY (username) REFERENCES codeharmony.users(username) ON DELETE CASCADE
);

CREATE TABLE codeharmony.published_lesson_plan (
	plan_name VARCHAR(128) NOT NULL,
	username VARCHAR(32) NOT NULL,
	description VARCHAR(300) NOT NULL DEFAULT '',

	CONSTRAINT published_lesson_plan_pk PRIMARY KEY (plan_name,username),
	CONSTRAINT published_lesson_plan_username_fk FOREIGN KEY (username) REFERENCES codeharmony.users(username) ON DELETE CASCADE
);

CREATE TABLE codeharmony.lesson_session (
	session_date TIMESTAMP NOT NULL DEFAULT current_timestamp,
	session_name VARCHAR(128) NOT NULL,
	plan_name VARCHAR(128) NOT NULL,
    username VARCHAR(32) NOT NULL,
	
	CONSTRAINT lesson_session_pk PRIMARY KEY (session_name,plan_name,username),
	CONSTRAINT lesson_session_plan_name_fk FOREIGN KEY (plan_name,username) REFERENCES codeharmony.lesson_plan(plan_name,username) ON DELETE CASCADE
);

CREATE TABLE codeharmony.lesson_plan_section (
	plan_name VARCHAR(128) NOT NULL,
	username VARCHAR(32) NOT NULL,
	section_elements JSONB NOT null default '[]',
	order_pos int2 NOT NULL,
	coding_data JSONB NOT NULL DEFAULT '{"language": "python", "startingCode": "", "expectedOutput": ""}',
	section_name VARCHAR(64) NOT NULL,
	section_type CHAR(8) NOT NULL,

	CONSTRAINT lesson_plan_section_pk PRIMARY KEY (plan_name,username,section_name),
	CONSTRAINT lesson_plan_section_plan_fk FOREIGN KEY (username,plan_name) REFERENCES codeharmony.lesson_plan(username,plan_name) ON DELETE CASCADE,
	CONSTRAINT plan_section_name_length CHECK (length(section_name) >= 1)
);

CREATE TABLE codeharmony.published_lesson_plan_section (
	plan_name VARCHAR(128) NOT NULL,
	username VARCHAR(32) NOT NULL,
	section_elements JSONB NOT null default '[]',
	order_pos int2 NOT NULL,
	coding_data JSONB NOT NULL DEFAULT '{"language": "python", "startingCode": "", "expectedOutput": ""}',
	section_name VARCHAR(64) NOT NULL,
	section_type CHAR(8) NOT NULL,

	CONSTRAINT published_lesson_plan_section_pk PRIMARY KEY (plan_name,username,section_name),
	CONSTRAINT published_lesson_plan_section_plan_fk FOREIGN KEY (username,plan_name) REFERENCES codeharmony.published_lesson_plan(username,plan_name) ON DELETE CASCADE,
	CONSTRAINT published_plan_section_name_length CHECK (length(section_name) >= 1)
);

CREATE TABLE codeharmony.student_teacher(
	student_un VARCHAR (32) NOT NULL,
	teacher_un VARCHAR (32) NOT NULL,

	CONSTRAINT student_teacher_pk PRIMARY KEY (student_un, teacher_un),
	CONSTRAINT teacher_un_fk FOREIGN KEY (teacher_un) REFERENCES codeharmony.users(username) ON DELETE CASCADE, 
	CONSTRAINT student_un_fk FOREIGN KEY (student_un) REFERENCES codeharmony.users(username) ON DELETE CASCADE
);

CREATE TABLE codeharmony.code_submission(
	teacher_un VARCHAR (32) NOT NULL,
	plan_name VARCHAR(128) NOT NULL,
	section_name VARCHAR(64) NOT NULL,
	session_name VARCHAR(128) NOT NULL,
	student_un VARCHAR (32) NOT NULL,
	code TEXT NOT NULL DEFAULT '',
	correct BOOLEAN NOT NULL DEFAULT false,
	CONSTRAINT code_submission_pk PRIMARY KEY (teacher_un, plan_name, section_name, session_name, student_un),
	CONSTRAINT code_submission_plan_fk FOREIGN KEY (teacher_un,plan_name,section_name) REFERENCES codeharmony.lesson_plan_section(username,plan_name,section_name) ON DELETE CASCADE,
	CONSTRAINT code_submission_session_fk FOREIGN KEY (plan_name, session_name, teacher_un) REFERENCES codeharmony.lesson_session(plan_name, session_name, username) ON DELETE CASCADE,
	CONSTRAINT code_submission_student_teacher_fk FOREIGN KEY (teacher_un, student_un) REFERENCES codeharmony.student_teacher(teacher_un, student_un) ON DELETE CASCADE
);

INSERT INTO codeharmony.users (username,hash,email) VALUES('user1','$argon2id$v=19$m=4096,t=3,p=1$mthVV+FY4YjPzyui4crAUA$0Hp4NgFmf4fLJzrtyitrUYUxL07HDCMax0/9HX9TEps','zacxalot@gmail.com');
--INSERT INTO codeharmony.users (username) VALUES('SamG');
--INSERT INTO codeharmony.users (username) VALUES('AlG');
--INSERT INTO codeharmony.users (username) VALUES('FergieF');
--INSERT INTO codeharmony.student_teacher (student_un, teacher_un) VALUES('SamG', 'user1');
INSERT INTO codeharmony.lesson_plan (plan_name, username) VALUES('Python 101', 'user1');
INSERT INTO codeharmony.lesson_plan_section (plan_name,username,section_elements,section_name,section_type, order_pos, coding_data) 
VALUES('Python 101','user1',
'[
	{"elType":"Typography","props":{"variant":"h1"},"children":{"String":"Welcome to Python"}},
	{"elType":"Typography","props":{"variant":"p"},"children":{"String":"It a snake of a language"}}
]',
'Introduction',
'LECTURE',
0,
'{"language":"python", "startingCode":"", "expectedOutput":""}');

INSERT INTO codeharmony.lesson_plan_section (plan_name,username,section_elements,section_name,section_type, order_pos, coding_data) 
VALUES('Python 101','user1',
'[
	{"elType":"Typography","props":{"variant":"p"},"children":{"String":"See if you can modify the starting code to print out \"hello world\""}}
]',
'Hello World',
'CODING',
1,
'{"language":"python", "startingCode":"print(\"Hi Python\")", "expectedOutput":"hello world"}');

