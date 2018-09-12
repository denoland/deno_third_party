"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var toAbsoluteGlob = require("@dsherret/to-absolute-glob");
var path = require("path");
var constants_1 = require("../constants");
var ArrayUtils_1 = require("./ArrayUtils");
var StringUtils_1 = require("./StringUtils");
var globParent = require("glob-parent");
var isNegatedGlob = require("is-negated-glob");
var FileUtils = /** @class */ (function () {
    function FileUtils() {
    }
    /**
     * Gets if the error is a file not found or directory not found error.
     * @param err - Error to check.
     */
    FileUtils.isNotExistsError = function (err) {
        return err.code === FileUtils.ENOENT;
    };
    /**
     * Joins the paths.
     * @param paths - Paths to join.
     */
    FileUtils.pathJoin = function () {
        var paths = [];
        for (var _i = 0; _i < arguments.length; _i++) {
            paths[_i] = arguments[_i];
        }
        return FileUtils.standardizeSlashes(path.join.apply(path, tslib_1.__spread(paths)));
    };
    /**
     * Gets if the path is absolute.
     * @param fileOrDirPath - File or directory path.
     */
    FileUtils.pathIsAbsolute = function (fileOrDirPath) {
        return path.isAbsolute(fileOrDirPath);
    };
    /**
     * Gets the standardized absolute path.
     * @param fileSystem - File system.
     * @param fileOrDirPath - Path to standardize.
     * @param relativeBase - Base path to be relative from.
     */
    FileUtils.getStandardizedAbsolutePath = function (fileSystem, fileOrDirPath, relativeBase) {
        return FileUtils.standardizeSlashes(path.normalize(getAbsolutePath()));
        function getAbsolutePath() {
            var isAbsolutePath = path.isAbsolute(fileOrDirPath);
            if (isAbsolutePath)
                return fileOrDirPath;
            if (!StringUtils_1.StringUtils.startsWith(fileOrDirPath, "./") && relativeBase != null)
                return path.join(relativeBase, fileOrDirPath);
            return path.join(fileSystem.getCurrentDirectory(), fileOrDirPath);
        }
    };
    /**
     * Gets the directory path.
     * @param fileOrDirPath - Path to get the directory name from.
     */
    FileUtils.getDirPath = function (fileOrDirPath) {
        return FileUtils.standardizeSlashes(path.dirname(fileOrDirPath));
    };
    /**
     * Gets the base name.
     * @param fileOrDirPath - Path to get the base name from.
     */
    FileUtils.getBaseName = function (fileOrDirPath) {
        return path.basename(fileOrDirPath);
    };
    /**
     * Gets the extension of the file name.
     * @param fileOrDirPath - Path to get the extension from.
     */
    FileUtils.getExtension = function (fileOrDirPath) {
        var baseName = FileUtils.getBaseName(fileOrDirPath);
        var lastDotIndex = baseName.lastIndexOf(".");
        if (lastDotIndex <= 0) // for files like .gitignore, need to include 0
            return ""; // same behaviour as node
        var lastExt = baseName.substring(lastDotIndex);
        var lastExtLowerCase = lastExt.toLowerCase();
        if (lastExtLowerCase === ".ts" && baseName.substring(lastDotIndex - 2, lastDotIndex).toLowerCase() === ".d")
            return baseName.substring(lastDotIndex - 2);
        if (lastExtLowerCase === ".map" && baseName.substring(lastDotIndex - 3, lastDotIndex).toLowerCase() === ".js")
            return baseName.substring(lastDotIndex - 3);
        return lastExt;
    };
    /**
     * Changes all back slashes to forward slashes.
     * @param fileOrDirPath - Path.
     */
    FileUtils.standardizeSlashes = function (fileOrDirPath) {
        return fileOrDirPath.replace(this.standardizeSlashesRegex, "/");
    };
    /**
     * Checks if a path ends with a specified search path.
     * @param fileOrDirPath - Path.
     * @param endsWithPath - Ends with path.
     */
    FileUtils.pathEndsWith = function (fileOrDirPath, endsWithPath) {
        var pathItems = FileUtils.splitPathBySlashes(fileOrDirPath);
        var endsWithItems = FileUtils.splitPathBySlashes(endsWithPath);
        if (endsWithItems.length > pathItems.length)
            return false;
        for (var i = 0; i < endsWithItems.length; i++) {
            if (endsWithItems[endsWithItems.length - i - 1] !== pathItems[pathItems.length - i - 1])
                return false;
        }
        return endsWithItems.length > 0;
    };
    /**
     * Checks if a path starts with a specified search path.
     * @param fileOrDirPath - Path.
     * @param startsWithPath - Starts with path.
     */
    FileUtils.pathStartsWith = function (fileOrDirPath, startsWithPath) {
        var isfileOrDirPathEmpty = StringUtils_1.StringUtils.isNullOrWhitespace(fileOrDirPath);
        var isStartsWithPathEmpty = StringUtils_1.StringUtils.isNullOrWhitespace(startsWithPath);
        var pathItems = FileUtils.splitPathBySlashes(fileOrDirPath);
        var startsWithItems = FileUtils.splitPathBySlashes(startsWithPath);
        if (isfileOrDirPathEmpty && isStartsWithPathEmpty)
            return true;
        if (isStartsWithPathEmpty || startsWithItems.length > pathItems.length)
            return false;
        // return true for the root directory
        if (startsWithItems.length === 1 && startsWithItems[0].length === 0)
            return true;
        for (var i = 0; i < startsWithItems.length; i++) {
            if (startsWithItems[i] !== pathItems[i])
                return false;
        }
        return startsWithItems.length > 0;
    };
    FileUtils.splitPathBySlashes = function (fileOrDirPath) {
        fileOrDirPath = (fileOrDirPath || "").replace(FileUtils.trimSlashStartRegex, "").replace(FileUtils.trimSlashEndRegex, "");
        return FileUtils.standardizeSlashes(fileOrDirPath).replace(/^\//, "").split("/");
    };
    /**
     * Gets the parent most paths out of the list of paths.
     * @param paths - File or directory paths.
     */
    FileUtils.getParentMostPaths = function (paths) {
        var e_1, _a;
        var finalPaths = [];
        var _loop_1 = function (fileOrDirPath) {
            if (finalPaths.every(function (p) { return !FileUtils.pathStartsWith(fileOrDirPath, p); }))
                finalPaths.push(fileOrDirPath);
        };
        try {
            for (var _b = tslib_1.__values(ArrayUtils_1.ArrayUtils.sortByProperty(paths, function (p) { return p.length; })), _c = _b.next(); !_c.done; _c = _b.next()) {
                var fileOrDirPath = _c.value;
                _loop_1(fileOrDirPath);
            }
        }
        catch (e_1_1) { e_1 = { error: e_1_1 }; }
        finally {
            try {
                if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
            }
            finally { if (e_1) throw e_1.error; }
        }
        return finalPaths;
    };
    /**
     * Reads a file or returns false if the file doesn't exist.
     * @param fileSystem - File System.
     * @param filePath - Path to file.
     * @param encoding - File encoding.
     */
    FileUtils.readFileOrNotExists = function (fileSystem, filePath, encoding) {
        return tslib_1.__awaiter(this, void 0, void 0, function () {
            var err_1;
            return tslib_1.__generator(this, function (_a) {
                switch (_a.label) {
                    case 0:
                        _a.trys.push([0, 2, , 3]);
                        return [4 /*yield*/, fileSystem.readFile(filePath, encoding)];
                    case 1: return [2 /*return*/, _a.sent()];
                    case 2:
                        err_1 = _a.sent();
                        if (!FileUtils.isNotExistsError(err_1))
                            throw err_1;
                        return [2 /*return*/, false];
                    case 3: return [2 /*return*/];
                }
            });
        });
    };
    /**
     * Reads a file synchronously or returns false if the file doesn't exist.
     * @param fileSystem - File System.
     * @param filePath - Path to file.
     * @param encoding - File encoding.
     */
    FileUtils.readFileOrNotExistsSync = function (fileSystem, filePath, encoding) {
        try {
            return fileSystem.readFileSync(filePath, encoding);
        }
        catch (err) {
            if (!FileUtils.isNotExistsError(err))
                throw err;
            return false;
        }
    };
    /**
     * Gets the text with a byte order mark.
     * @param text - Text.
     */
    FileUtils.getTextWithByteOrderMark = function (text) {
        if (text[0] === constants_1.Chars.BOM)
            return text;
        return constants_1.Chars.BOM + text;
    };
    /**
     * Gets the relative path from one absolute path to another.
     * @param absoluteDirPathFrom - Absolute directory path from.
     * @param absolutePathTo - Absolute path to.
     */
    FileUtils.getRelativePathTo = function (absoluteDirPathFrom, absolutePathTo) {
        var relativePath = path.relative(absoluteDirPathFrom, path.dirname(absolutePathTo));
        return FileUtils.standardizeSlashes(path.join(relativePath, path.basename(absolutePathTo)));
    };
    /**
     * Gets if the path is for the root directory.
     * @param path - Path.
     */
    FileUtils.isRootDirPath = function (dirOrFilePath) {
        return dirOrFilePath === FileUtils.getDirPath(dirOrFilePath);
    };
    /**
     * Gets the descendant directories of the specified directory.
     * @param dirPath - Directory path.
     */
    FileUtils.getDescendantDirectories = function (fileSystemWrapper, dirPath) {
        // todo: unit tests...
        return Array.from(getDescendantDirectories(dirPath));
        function getDescendantDirectories(currentDirPath) {
            var e_2, _a, subDirPaths, subDirPaths_1, subDirPaths_1_1, subDirPath, e_2_1;
            return tslib_1.__generator(this, function (_b) {
                switch (_b.label) {
                    case 0:
                        subDirPaths = fileSystemWrapper.readDirSync(currentDirPath).filter(function (d) { return fileSystemWrapper.directoryExistsSync(d); });
                        _b.label = 1;
                    case 1:
                        _b.trys.push([1, 7, 8, 9]);
                        subDirPaths_1 = tslib_1.__values(subDirPaths), subDirPaths_1_1 = subDirPaths_1.next();
                        _b.label = 2;
                    case 2:
                        if (!!subDirPaths_1_1.done) return [3 /*break*/, 6];
                        subDirPath = subDirPaths_1_1.value;
                        return [4 /*yield*/, subDirPath];
                    case 3:
                        _b.sent();
                        return [5 /*yield**/, tslib_1.__values(getDescendantDirectories(subDirPath))];
                    case 4:
                        _b.sent();
                        _b.label = 5;
                    case 5:
                        subDirPaths_1_1 = subDirPaths_1.next();
                        return [3 /*break*/, 2];
                    case 6: return [3 /*break*/, 9];
                    case 7:
                        e_2_1 = _b.sent();
                        e_2 = { error: e_2_1 };
                        return [3 /*break*/, 9];
                    case 8:
                        try {
                            if (subDirPaths_1_1 && !subDirPaths_1_1.done && (_a = subDirPaths_1.return)) _a.call(subDirPaths_1);
                        }
                        finally { if (e_2) throw e_2.error; }
                        return [7 /*endfinally*/];
                    case 9: return [2 /*return*/];
                }
            });
        }
    };
    /**
     * Gets the glob as absolute.
     * @param glob - Glob.
     * @param cwd - Current working directory.
     */
    FileUtils.toAbsoluteGlob = function (glob, cwd) {
        return toAbsoluteGlob(glob, { cwd: cwd });
    };
    /**
     * Gets if the glob is a negated glob.
     * @param glob - Glob.
     */
    FileUtils.isNegatedGlob = function (glob) {
        return isNegatedGlob(glob).negated;
    };
    /**
     * Gets the glob's directory.
     * @param glob - Glob.
     */
    FileUtils.getGlobDir = function (glob) {
        return globParent(glob);
    };
    FileUtils.standardizeSlashesRegex = /\\/g;
    FileUtils.trimSlashStartRegex = /^\//;
    FileUtils.trimSlashEndRegex = /\/$/;
    FileUtils.ENOENT = "ENOENT";
    return FileUtils;
}());
exports.FileUtils = FileUtils;
