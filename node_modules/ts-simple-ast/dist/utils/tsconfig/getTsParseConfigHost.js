"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var tsInternal = require("../../typescript/tsInternal");
var utils_1 = require("../../utils");
function getTsParseConfigHost(fileSystemWrapper, options) {
    var directories = [];
    var currentDir = fileSystemWrapper.getCurrentDirectory();
    var useCaseSensitiveFileNames = false; // shouldn't this be true? (it was false like this in the compiler)
    var host = {
        useCaseSensitiveFileNames: useCaseSensitiveFileNames,
        readDirectory: function (rootDir, extensions, excludes, includes) {
            // start: code from compiler api
            var regexFlag = useCaseSensitiveFileNames ? "" : "i";
            var patterns = tsInternal.getFileMatcherPatterns(rootDir, excludes || [], includes, useCaseSensitiveFileNames, currentDir);
            var includeDirectoryRegex = patterns.includeDirectoryPattern && new RegExp(patterns.includeDirectoryPattern, regexFlag);
            var excludeRegex = patterns.excludePattern && new RegExp(patterns.excludePattern, regexFlag);
            // end
            return tsInternal.matchFiles(rootDir, extensions, excludes || [], includes, useCaseSensitiveFileNames, currentDir, undefined, function (path) {
                var includeDir = dirPathMatches(path);
                path = fileSystemWrapper.getStandardizedAbsolutePath(path);
                if (includeDir)
                    directories.push(path);
                return getFileSystemEntries(path, fileSystemWrapper);
            });
            function dirPathMatches(absoluteName) {
                // needed for the regex to match
                if (absoluteName[absoluteName.length - 1] !== "/")
                    absoluteName += "/";
                // condition is from compiler api
                return (!includeDirectoryRegex || includeDirectoryRegex.test(absoluteName))
                    && (!excludeRegex || !excludeRegex.test(absoluteName));
            }
        },
        fileExists: function (path) { return fileSystemWrapper.fileExistsSync(path); },
        readFile: function (path) { return fileSystemWrapper.readFileSync(path, options.encoding); },
        getDirectories: function () { return tslib_1.__spread(directories); },
        clearDirectories: function () { return directories.length = 0; }
    };
    return host;
}
exports.getTsParseConfigHost = getTsParseConfigHost;
function getFileSystemEntries(path, fileSystemWrapper) {
    var e_1, _a;
    var files = [];
    var directories = [];
    try {
        var entries = fileSystemWrapper.readDirSync(path);
        try {
            for (var entries_1 = tslib_1.__values(entries), entries_1_1 = entries_1.next(); !entries_1_1.done; entries_1_1 = entries_1.next()) {
                var entry = entries_1_1.value;
                if (fileSystemWrapper.fileExistsSync(entry))
                    files.push(entry);
                else
                    directories.push(entry);
            }
        }
        catch (e_1_1) { e_1 = { error: e_1_1 }; }
        finally {
            try {
                if (entries_1_1 && !entries_1_1.done && (_a = entries_1.return)) _a.call(entries_1);
            }
            finally { if (e_1) throw e_1.error; }
        }
    }
    catch (err) {
        if (!utils_1.FileUtils.isNotExistsError(err))
            throw err;
    }
    return { files: files, directories: directories };
}
