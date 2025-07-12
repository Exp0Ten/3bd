#include <unistd.h>
#include <sys/ptrace.h>

extern char **environ;

int main(int argc, char* argv[]) {

    ptrace(PT_TRACE_ME); //trace me request

    char * path = argv[1];
    char ** arguments = &argv[1];
    execve(path, arguments, environ); //runs the file at path (first argument)

    return 1; // If exec call is incorrect, then return exit code 1
}