// lang.System

package java.lang

import java.io.PrintStream

object System {
    val out = PrintStream()

    @JvmStatic
    fun currentTimeMillis(): Long = kotlin.system.getTimeMillis()
}
