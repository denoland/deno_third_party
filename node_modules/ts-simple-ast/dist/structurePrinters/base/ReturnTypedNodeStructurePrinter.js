"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var codeBlockWriter_1 = require("../../codeBlockWriter");
var utils_1 = require("../../utils");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var ReturnTypedNodeStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(ReturnTypedNodeStructurePrinter, _super);
    function ReturnTypedNodeStructurePrinter(factory, alwaysWrite) {
        if (alwaysWrite === void 0) { alwaysWrite = false; }
        var _this = _super.call(this, factory) || this;
        _this.alwaysWrite = alwaysWrite;
        return _this;
    }
    ReturnTypedNodeStructurePrinter.prototype.printText = function (writer, structure) {
        var returnType = structure.returnType;
        if (returnType == null && this.alwaysWrite === false)
            return;
        returnType = returnType || "void";
        // todo: hacky, will need to change this in the future...
        var initializerText = typeof returnType === "string" ? returnType : getTextForWriterFunc(returnType);
        if (!utils_1.StringUtils.isNullOrWhitespace(initializerText))
            writer.write(": " + initializerText);
        function getTextForWriterFunc(writerFunc) {
            var newWriter = new codeBlockWriter_1.CodeBlockWriter(writer.getOptions());
            writerFunc(newWriter);
            return newWriter.toString();
        }
    };
    return ReturnTypedNodeStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.ReturnTypedNodeStructurePrinter = ReturnTypedNodeStructurePrinter;
