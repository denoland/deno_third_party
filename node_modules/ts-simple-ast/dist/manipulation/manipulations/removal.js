"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var typescript_1 = require("../../typescript");
var formatting_1 = require("../formatting");
var nodeHandlers_1 = require("../nodeHandlers");
var textManipulators_1 = require("../textManipulators");
var doManipulation_1 = require("./doManipulation");
function removeChildren(opts) {
    var children = opts.children;
    if (children.length === 0)
        return;
    doManipulation_1.doManipulation(children[0].getSourceFile(), new textManipulators_1.RemoveChildrenTextManipulator(opts), new nodeHandlers_1.NodeHandlerFactory().getForChildIndex({
        parent: children[0].getParentSyntaxList() || children[0].getParentOrThrow(),
        childIndex: children[0].getChildIndex(),
        childCount: -1 * children.length
    }));
}
exports.removeChildren = removeChildren;
function removeChildrenWithFormattingFromCollapsibleSyntaxList(opts) {
    var children = opts.children;
    if (children.length === 0)
        return;
    var syntaxList = children[0].getParentSyntaxListOrThrow();
    if (syntaxList.getChildCount() === children.length) {
        removeChildrenWithFormatting({
            children: [syntaxList],
            getSiblingFormatting: function () { return formatting_1.FormattingKind.None; }
        });
    }
    else
        removeChildrenWithFormatting(opts);
}
exports.removeChildrenWithFormattingFromCollapsibleSyntaxList = removeChildrenWithFormattingFromCollapsibleSyntaxList;
function removeChildrenWithFormatting(opts) {
    var children = opts.children, getSiblingFormatting = opts.getSiblingFormatting;
    if (children.length === 0)
        return;
    doManipulation_1.doManipulation(children[0].sourceFile, new textManipulators_1.RemoveChildrenWithFormattingTextManipulator({
        children: children,
        getSiblingFormatting: getSiblingFormatting
    }), new nodeHandlers_1.NodeHandlerFactory().getForChildIndex({
        parent: children[0].getParentSyntaxList() || children[0].getParentOrThrow(),
        childIndex: children[0].getChildIndex(),
        childCount: -1 * children.length
    }));
}
exports.removeChildrenWithFormatting = removeChildrenWithFormatting;
function removeOverloadableClassMember(classMember) {
    if (classMember.isOverload()) {
        if (classMember.getParentOrThrow().isAmbient())
            removeClassMember(classMember);
        else
            removeChildren({ children: [classMember], removeFollowingSpaces: true, removeFollowingNewLines: true });
    }
    else
        removeClassMembers(tslib_1.__spread(classMember.getOverloads(), [classMember]));
}
exports.removeOverloadableClassMember = removeOverloadableClassMember;
function removeClassMember(classMember) {
    removeClassMembers([classMember]);
}
exports.removeClassMember = removeClassMember;
function removeClassMembers(classMembers) {
    removeChildrenWithFormatting({
        getSiblingFormatting: formatting_1.getClassMemberFormatting,
        children: classMembers
    });
}
exports.removeClassMembers = removeClassMembers;
function removeInterfaceMember(classMember) {
    removeInterfaceMembers([classMember]);
}
exports.removeInterfaceMember = removeInterfaceMember;
function removeInterfaceMembers(classMembers) {
    removeChildrenWithFormatting({
        getSiblingFormatting: formatting_1.getInterfaceMemberFormatting,
        children: classMembers
    });
}
exports.removeInterfaceMembers = removeInterfaceMembers;
function removeCommaSeparatedChild(child) {
    var childrenToRemove = [child];
    var syntaxList = child.getParentSyntaxListOrThrow();
    var isRemovingFirstChild = childrenToRemove[0] === syntaxList.getFirstChild();
    addNextCommaIfAble();
    addPreviousCommaIfAble();
    removeChildren({
        children: childrenToRemove,
        removePrecedingSpaces: !isRemovingFirstChild,
        removeFollowingSpaces: isRemovingFirstChild,
        removePrecedingNewLines: !isRemovingFirstChild,
        removeFollowingNewLines: isRemovingFirstChild
    });
    function addNextCommaIfAble() {
        var commaToken = child.getNextSiblingIfKind(typescript_1.SyntaxKind.CommaToken);
        if (commaToken != null)
            childrenToRemove.push(commaToken);
    }
    function addPreviousCommaIfAble() {
        if (syntaxList.getLastChild() !== childrenToRemove[childrenToRemove.length - 1])
            return;
        var precedingComma = child.getPreviousSiblingIfKind(typescript_1.SyntaxKind.CommaToken);
        if (precedingComma != null)
            childrenToRemove.unshift(precedingComma);
    }
}
exports.removeCommaSeparatedChild = removeCommaSeparatedChild;
function removeOverloadableStatementedNodeChild(node) {
    if (node.isOverload())
        removeChildren({ children: [node], removeFollowingSpaces: true, removeFollowingNewLines: true });
    else
        removeStatementedNodeChildren(tslib_1.__spread(node.getOverloads(), [node]));
}
exports.removeOverloadableStatementedNodeChild = removeOverloadableStatementedNodeChild;
function removeStatementedNodeChild(node) {
    removeStatementedNodeChildren([node]);
}
exports.removeStatementedNodeChild = removeStatementedNodeChild;
function removeStatementedNodeChildren(nodes) {
    removeChildrenWithFormatting({
        getSiblingFormatting: formatting_1.getStatementedNodeChildFormatting,
        children: nodes
    });
}
exports.removeStatementedNodeChildren = removeStatementedNodeChildren;
function removeClausedNodeChild(node) {
    removeClausedNodeChildren([node]);
}
exports.removeClausedNodeChild = removeClausedNodeChild;
function removeClausedNodeChildren(nodes) {
    removeChildrenWithFormatting({
        getSiblingFormatting: formatting_1.getClausedNodeChildFormatting,
        children: nodes
    });
}
exports.removeClausedNodeChildren = removeClausedNodeChildren;
function unwrapNode(node) {
    doManipulation_1.doManipulation(node.sourceFile, new textManipulators_1.UnwrapTextManipulator(node), new nodeHandlers_1.NodeHandlerFactory().getForUnwrappingNode(node));
}
exports.unwrapNode = unwrapNode;
