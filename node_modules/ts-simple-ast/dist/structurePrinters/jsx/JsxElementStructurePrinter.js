"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var JsxElementStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(JsxElementStructurePrinter, _super);
    function JsxElementStructurePrinter() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    JsxElementStructurePrinter.prototype.printText = function (writer, structure) {
        writer.write("<" + structure.name);
        if (structure.attributes)
            this.printAttributes(writer, structure.attributes);
        if (this.isSelfClosing(structure)) {
            writer.write(" />");
            return;
        }
        writer.write(">");
        if (structure.children != null)
            this.printChildren(writer, structure.children);
        writer.write("</" + structure.name + ">");
    };
    JsxElementStructurePrinter.prototype.isSelfClosing = function (structure) {
        if (structure.isSelfClosing === true)
            return true;
        return structure.isSelfClosing == null && structure.children == null;
    };
    JsxElementStructurePrinter.prototype.printAttributes = function (writer, attributes) {
        var e_1, _a;
        var attributePrinter = this.factory.forJsxAttribute();
        try {
            for (var attributes_1 = tslib_1.__values(attributes), attributes_1_1 = attributes_1.next(); !attributes_1_1.done; attributes_1_1 = attributes_1.next()) {
                var attrib = attributes_1_1.value;
                writer.space();
                attributePrinter.printText(writer, attrib);
            }
        }
        catch (e_1_1) { e_1 = { error: e_1_1 }; }
        finally {
            try {
                if (attributes_1_1 && !attributes_1_1.done && (_a = attributes_1.return)) _a.call(attributes_1);
            }
            finally { if (e_1) throw e_1.error; }
        }
    };
    JsxElementStructurePrinter.prototype.printChildren = function (writer, children) {
        var _this = this;
        writer.newLine();
        writer.indentBlock(function () {
            var e_2, _a;
            try {
                for (var children_1 = tslib_1.__values(children), children_1_1 = children_1.next(); !children_1_1.done; children_1_1 = children_1.next()) {
                    var child = children_1_1.value;
                    _this.printText(writer, child);
                    writer.newLine();
                }
            }
            catch (e_2_1) { e_2 = { error: e_2_1 }; }
            finally {
                try {
                    if (children_1_1 && !children_1_1.done && (_a = children_1.return)) _a.call(children_1);
                }
                finally { if (e_2) throw e_2.error; }
            }
        });
    };
    return JsxElementStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.JsxElementStructurePrinter = JsxElementStructurePrinter;
