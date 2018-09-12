"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var manipulation_1 = require("../../manipulation");
var structurePrinters_1 = require("../../structurePrinters");
var typescript_1 = require("../../typescript");
var callBaseFill_1 = require("../callBaseFill");
function ImplementsClauseableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.getImplements = function () {
            var implementsClause = this.getHeritageClauseByKind(typescript_1.SyntaxKind.ImplementsKeyword);
            return implementsClause == null ? [] : implementsClause.getTypeNodes();
        };
        class_1.prototype.addImplements = function (text) {
            return this.insertImplements(this.getImplements().length, text);
        };
        class_1.prototype.insertImplements = function (index, texts) {
            var length = texts instanceof Array ? texts.length : 0;
            if (typeof texts === "string") {
                errors.throwIfNotStringOrWhitespace(texts, "texts");
                texts = [texts];
            }
            else if (texts.length === 0) {
                return [];
            }
            var writer = this.getWriterWithQueuedChildIndentation();
            var structurePrinter = new structurePrinters_1.CommaSeparatedStructuresPrinter(new structurePrinters_1.StringStructurePrinter());
            structurePrinter.printText(writer, texts);
            var heritageClauses = this.getHeritageClauses();
            var implementsTypes = this.getImplements();
            index = manipulation_1.verifyAndGetIndex(index, implementsTypes.length);
            if (implementsTypes.length > 0) {
                var implementsClause = this.getHeritageClauseByKindOrThrow(typescript_1.SyntaxKind.ImplementsKeyword);
                manipulation_1.insertIntoCommaSeparatedNodes({
                    parent: implementsClause.getFirstChildByKindOrThrow(typescript_1.SyntaxKind.SyntaxList),
                    currentNodes: implementsTypes,
                    insertIndex: index,
                    newText: writer.toString()
                });
                return manipulation_1.getNodeOrNodesToReturn(this.getImplements(), index, length);
            }
            var openBraceToken = this.getFirstChildByKindOrThrow(typescript_1.SyntaxKind.OpenBraceToken);
            var openBraceStart = openBraceToken.getStart();
            var isLastSpace = /\s/.test(this.getSourceFile().getFullText()[openBraceStart - 1]);
            var insertText = "implements " + writer.toString() + " ";
            if (!isLastSpace)
                insertText = " " + insertText;
            // assumes there can only be another extends heritage clause
            manipulation_1.insertIntoParentTextRange({
                parent: heritageClauses.length === 0 ? this : heritageClauses[0].getParentSyntaxListOrThrow(),
                insertPos: openBraceStart,
                newText: insertText
            });
            return manipulation_1.getNodeOrNodesToReturn(this.getImplements(), index, length);
        };
        class_1.prototype.removeImplements = function (implementsNodeOrIndex) {
            var implementsClause = this.getHeritageClauseByKind(typescript_1.SyntaxKind.ImplementsKeyword);
            if (implementsClause == null)
                throw new errors.InvalidOperationError("Cannot remove an implements when none exist.");
            implementsClause.removeExpression(implementsNodeOrIndex);
            return this;
        };
        class_1.prototype.fill = function (structure) {
            callBaseFill_1.callBaseFill(Base.prototype, this, structure);
            if (structure.implements != null && structure.implements.length > 0)
                this.addImplements(structure.implements);
            return this;
        };
        return class_1;
    }(Base));
}
exports.ImplementsClauseableNode = ImplementsClauseableNode;
