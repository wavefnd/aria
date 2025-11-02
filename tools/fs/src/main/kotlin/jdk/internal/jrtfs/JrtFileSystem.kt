package jdk.internal.jrtfs

import java.nio.file.FileSystem
import java.nio.file.FileSystems
import java.nio.file.Path

class JrtFileSystem(private val provider: JrtFileSystemProvider) : FileSystem() {
    override fun provider() = provider
    override fun getSeparator() = "/"
    override fun getPath(first: String?, vararg more: String?): Path =
        FileSystems.getDefault().getPath(first, *more)
    override fun isOpen() = true
    override fun isReadOnly() = true
    override fun close() {}
    override fun supportedFileAttributeViews(): MutableSet<String> = mutableSetOf()
    override fun getRootDirectories(): MutableIterable<Path> = mutableListOf()
    override fun getFileStores(): MutableIterable<java.nio.file.FileStore> = mutableListOf()
    override fun getUserPrincipalLookupService() = FileSystems.getDefault().userPrincipalLookupService
    override fun getPathMatcher(syntaxAndPattern: String) = FileSystems.getDefault().getPathMatcher(syntaxAndPattern)
    override fun newWatchService() = FileSystems.getDefault().newWatchService()
}
