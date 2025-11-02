package jdk.internal.jrtfs

import java.net.URI
import java.nio.channels.SeekableByteChannel
import java.nio.file.*
import java.nio.file.attribute.*
import java.nio.file.spi.FileSystemProvider

class JrtFileSystemProvider : FileSystemProvider() {
    override fun getScheme(): String = "jrt"

    override fun newFileSystem(uri: URI?, env: MutableMap<String, *>?): FileSystem =
        JrtFileSystem(this)

    override fun getFileSystem(uri: URI?): FileSystem =
        JrtFileSystem(this)

    override fun getPath(uri: URI?): Path {
        val path = uri?.schemeSpecificPart ?: throw IllegalArgumentException("Invalid URI: $uri")
        return Paths.get(path)
    }

    override fun newByteChannel(
        path: Path,
        options: MutableSet<out OpenOption>,
        vararg attrs: FileAttribute<*>
    ): SeekableByteChannel {
        throw UnsupportedOperationException("jrt:/ filesystem is read-only")
    }

    override fun newDirectoryStream(dir: Path, filter: DirectoryStream.Filter<in Path>): DirectoryStream<Path> {
        throw UnsupportedOperationException("Not implemented")
    }

    override fun createDirectory(dir: Path, vararg attrs: FileAttribute<*>) {
        throw UnsupportedOperationException("Read-only")
    }

    override fun delete(path: Path) {
        throw UnsupportedOperationException("Read-only")
    }

    override fun copy(source: Path, target: Path, vararg options: CopyOption) {
        throw UnsupportedOperationException("Read-only")
    }

    override fun move(source: Path, target: Path, vararg options: CopyOption) {
        throw UnsupportedOperationException("Read-only")
    }

    override fun isSameFile(path: Path, path2: Path): Boolean = path == path2

    override fun isHidden(path: Path): Boolean = false

    override fun getFileStore(path: Path): FileStore {
        throw UnsupportedOperationException("No FileStore for jrt:/")
    }

    override fun checkAccess(path: Path, vararg modes: AccessMode) {
        // no-op (read-only)
    }

    override fun <V : FileAttributeView> getFileAttributeView(
        path: Path,
        type: Class<V>,
        vararg options: LinkOption
    ): V? = null

    override fun <A : BasicFileAttributes> readAttributes(
        path: Path,
        type: Class<A>,
        vararg options: LinkOption
    ): A {
        throw UnsupportedOperationException("readAttributes(Class) not implemented for jrt:/")
    }

    override fun readAttributes(
        path: Path,
        attributes: String,
        vararg options: LinkOption
    ): MutableMap<String, Any?> {
        return mutableMapOf()
    }

    override fun setAttribute(path: Path, attribute: String, value: Any?, vararg options: LinkOption) {
        throw UnsupportedOperationException("Read-only")
    }
}
