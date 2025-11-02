package jdk.internal.jrtfs

import java.nio.file.Path
import java.nio.file.FileSystems

class JrtPath(private val path: String) : Path by FileSystems.getDefault().getPath(path)
