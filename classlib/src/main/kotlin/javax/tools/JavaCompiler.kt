package javax.tools

/**
 * Minimal stub of JavaCompiler for AriaJDK.
 * This class exists only for IDE/JDK recognition and will be replaced
 * by the real compiler frontend (AriaC) in later versions.
 */
interface JavaCompiler {
    /**
     * Represents a compilation task.
     */
    interface CompilationTask {
        fun call(): Boolean
    }

    /**
     * Placeholder for actual compile method.
     * @param sources List of source files to compile.
     * @return a CompilationTask (stubbed)
     */
    fun getTask(
        out: Appendable? = null,
        fileManager: Any? = null,
        diagnosticListener: Any? = null,
        options: List<String>? = null,
        classes: Iterable<String>? = null,
        compilationUnits: Iterable<String>? = null
    ): CompilationTask {
        return object : CompilationTask {
            override fun call(): Boolean {
                println("⚙️ [AriaJDK] JavaCompiler stub invoked. (No real compilation yet)")
                return true
            }
        }
    }
}
