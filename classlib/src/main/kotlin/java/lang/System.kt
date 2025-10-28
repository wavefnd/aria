// lang.System

package java.lang

object System {
    val out = PrintStream()

    @JvmStatic
    fun currentTimeMillis(): Long = kotlin.system.getTimeMillis()
}
